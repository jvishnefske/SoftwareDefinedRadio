/**
 * SDR DSP AudioWorklet Processor
 *
 * Runs Rust WASM DSP code in the AudioWorklet thread for
 * real-time IQ processing with low latency.
 */

class SdrDspProcessor extends AudioWorkletProcessor {
    constructor(options) {
        super();

        this.wasmReady = false;
        this.wasmInstance = null;
        this.wasmExports = null;
        this.dspProcessor = null;
        this.spectrumBuffer = null;
        this.spectrumView = null;
        this.frameCount = 0;

        // Handle messages from main thread
        this.port.onmessage = (event) => this.handleMessage(event.data);
    }

    async handleMessage(data) {
        switch (data.type) {
            case 'init':
                await this.initWasm(data.wasmModule, data.spectrumBuffer);
                break;

            case 'setMode':
                if (this.wasmExports && this.dspProcessor) {
                    this.wasmExports.set_mode(this.dspProcessor, data.mode);
                }
                break;

            case 'setFrequency':
                if (this.wasmExports && this.dspProcessor) {
                    this.wasmExports.set_frequency_offset(this.dspProcessor, data.offsetHz);
                }
                break;

            case 'setFilter':
                if (this.wasmExports && this.dspProcessor) {
                    this.wasmExports.set_filter_bandwidth(this.dspProcessor, data.bandwidth);
                }
                break;

            case 'setAgc':
                if (this.wasmExports && this.dspProcessor) {
                    this.wasmExports.set_agc(
                        this.dspProcessor,
                        data.attack_ms,
                        data.decay_ms,
                        data.hang_ms
                    );
                }
                break;

            case 'reset':
                if (this.wasmExports && this.dspProcessor) {
                    this.wasmExports.reset(this.dspProcessor);
                }
                break;
        }
    }

    async initWasm(wasmModule, spectrumBuffer) {
        try {
            // Minimal imports for WASM
            const imports = {
                env: {
                    // Math functions (micromath should provide these, but just in case)
                    sinf: Math.sin,
                    cosf: Math.cos,
                    sqrtf: Math.sqrt,
                    log10f: Math.log10,
                    powf: Math.pow,
                    expf: Math.exp,
                    atan2f: Math.atan2,
                    fabsf: Math.abs,
                    floorf: Math.floor,
                    ceilf: Math.ceil,
                }
            };

            // Instantiate WASM module
            this.wasmInstance = await WebAssembly.instantiate(wasmModule, imports);
            this.wasmExports = this.wasmInstance.exports;

            // Create DSP processor instance
            this.dspProcessor = this.wasmExports.create_processor(sampleRate);

            // Setup SharedArrayBuffer for spectrum data
            if (spectrumBuffer) {
                this.spectrumBuffer = spectrumBuffer;
                this.spectrumView = new Float32Array(spectrumBuffer);
            }

            this.wasmReady = true;
            this.port.postMessage({ type: 'ready' });

        } catch (error) {
            this.port.postMessage({
                type: 'error',
                message: `WASM init failed: ${error.message}`
            });
        }
    }

    process(inputs, outputs, parameters) {
        // Skip if WASM not ready or no input
        if (!this.wasmReady || inputs[0].length === 0) {
            return true;
        }

        const input = inputs[0];
        const output = outputs[0];
        const numSamples = input[0]?.length || 128;

        // Get I and Q channels (stereo input: L=I, R=Q)
        const iChannel = input[0] || new Float32Array(numSamples);
        const qChannel = input[1] || new Float32Array(numSamples);

        // Get WASM buffer pointers
        const inputPtr = this.wasmExports.get_input_buffer_ptr(this.dspProcessor);
        const outputPtr = this.wasmExports.get_output_buffer_ptr(this.dspProcessor);
        const spectrumPtr = this.wasmExports.get_spectrum_buffer_ptr(this.dspProcessor);

        // Copy input to WASM memory (interleaved I/Q)
        const wasmMemory = new Float32Array(this.wasmExports.memory.buffer);
        const inputOffset = inputPtr / 4; // Convert byte offset to f32 index

        for (let i = 0; i < numSamples; i++) {
            wasmMemory[inputOffset + i * 2] = iChannel[i];
            wasmMemory[inputOffset + i * 2 + 1] = qChannel[i];
        }

        // Process audio through WASM DSP
        this.wasmExports.process(this.dspProcessor, numSamples);

        // Copy output from WASM memory (mono audio)
        const outputOffset = outputPtr / 4;
        for (let i = 0; i < numSamples; i++) {
            const sample = wasmMemory[outputOffset + i];
            if (output[0]) output[0][i] = sample;
            if (output[1]) output[1][i] = sample; // Duplicate to both channels
        }

        // Copy spectrum data to SharedArrayBuffer every 8 frames (~21ms at 48kHz)
        this.frameCount++;
        if (this.frameCount >= 8 && this.spectrumView) {
            this.frameCount = 0;

            const spectrumOffset = spectrumPtr / 4;
            const spectrumSize = 512;

            for (let i = 0; i < spectrumSize; i++) {
                this.spectrumView[i] = wasmMemory[spectrumOffset + i];
            }

            // Send S-meter update
            const smeter = this.wasmExports.get_smeter(this.dspProcessor);
            this.port.postMessage({ type: 'smeter', value: smeter });
        }

        return true; // Keep processor alive
    }
}

// Register the processor
registerProcessor('sdr-dsp-processor', SdrDspProcessor);
