MEMORY
{
  /* NOTE K = KiBi = 1024 bytes */
  FLASH : ORIGIN = 0x08000000, LENGTH = 16K
  RAM : ORIGIN = 0x20000000, LENGTH = 2K
}

/* We have 4K RAM, and only let the linker see 2K, so we have 2K reserved for stack. */
_stack_start = ORIGIN(RAM) + 4K;
