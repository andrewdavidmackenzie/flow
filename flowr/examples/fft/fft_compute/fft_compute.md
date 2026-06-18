# fft_compute

Computes the Discrete Fourier Transform of an input signal and returns
the dominant frequencies.

## Inputs
- `signal`: array of numbers (the time-domain samples)
- `sample_rate`: number (samples per second)

## Outputs
- `text`: a string with an ASCII bar chart of the frequency spectrum
- `bins`: array of numbers — the magnitude at each frequency bin (0 to Nyquist), suitable for feeding into `lib://flowstdlib/charts/histogram` for visualization
