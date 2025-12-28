/* Memory layout for STM32G474RE */
/* 512KB Flash, 128KB RAM */

MEMORY
{
    /* FLASH memory starts at 0x08000000 */
    /* Reserve first 16KB for bootloader */
    FLASH  : ORIGIN = 0x08000000, LENGTH = 512K

    /* RAM starts at 0x20000000 */
    /* 128KB total SRAM */
    RAM    : ORIGIN = 0x20000000, LENGTH = 128K

    /* CCM SRAM (core-coupled memory) for fast DSP buffers */
    /* 32KB at 0x10000000 - accessible only by CPU */
    CCMRAM : ORIGIN = 0x10000000, LENGTH = 32K
}

/* Memory sections */
SECTIONS
{
    /* DSP buffers in CCM for deterministic access */
    .dsp_buffers (NOLOAD) : {
        . = ALIGN(4);
        *(.dsp_buffers)
        *(.dsp_buffers.*)
        . = ALIGN(4);
    } > CCMRAM

    /* DMA buffers must be in regular SRAM (not CCM) */
    .dma_buffers (NOLOAD) : {
        . = ALIGN(4);
        *(.dma_buffers)
        *(.dma_buffers.*)
        . = ALIGN(4);
    } > RAM
}

/* Stack size */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);
