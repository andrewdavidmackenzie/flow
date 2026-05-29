## image_read

Read an image file (PNG) and output its pixel data as a flat array of
grayscale values (0-255) along with the image dimensions.

The image is converted to grayscale using the standard luminance formula:
`gray = 0.299*R + 0.587*G + 0.114*B`
