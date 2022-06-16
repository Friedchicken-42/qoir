
# qoir

A QOIF (Quite Ok Image Format) implementation that better compress pixel with the same operation.


## Optimizations

The **QOI_OP_RUN** operation is used with the previous operation instead of the previous pixel.

This way images with gradients can be better compress using one **QOI_OP_LUMA** or **QOI_OP_INDEX**. without needing multiple QOI_OP_RGB.

### Example

<p align="center">
  <img src="https://raw.githubusercontent.com/Friedchicken-42/qoir/main/static/a.png" width="200">
</p>

This image with the original format would be:

<table align="center"><tbody>
  <tr>
    <td>header</td>
    <td>rbg #ff0000</td>
    <td>rgb #fe0000</td>
    <td>... x253 </td>
    <td>rgb #000000</td>
  </tr>
  <tr>
    <td colspan=6 align="center"> ~1 kb </td>
  </tr>
</tbody></table>

with this implementation is reduced to:

<table align="center"><tbody>
  <tr>
    <td>header</td>
    <td>diff 1 -2 -2</td>
    <td>run 62</td>
    <td>run 62</td>
    <td>run 62</td>
    <td>run 62</td>
    <td>run 6</td>
  </tr>
  <tr>
    <td colspan=7 align="center"> 66 byte </td>
  </tr>
</tbody></table>

## TODO
- [ ]  extract code as a library
- [ ]  implement png <-> byte array in rust

## Acknowledgements

 - [qoiformat.org](https://qoiformat.org/)

