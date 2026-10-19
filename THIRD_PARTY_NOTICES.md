# Third-party notices

Hydragen's project license does not replace the applicable terms for third-party
material. This file preserves notices for OpenSpec content and Feather icons
included in this repository; it is not a complete notice bundle for compiled
Hydragen releases.

## OpenSpec commands and skills

The OpenSpec-derived commands in `.opencode/commands/opsx-*.md` and skills in
`.opencode/skills/openspec-*/SKILL.md` are provided under the MIT License. Their
existing license metadata remains in place. The notice below comes from
[OpenSpec](https://github.com/Fission-AI/OpenSpec/blob/main/LICENSE) and matches
the installed OpenSpec 1.5.0 package used to generate these files.

```text
MIT License

Copyright (c) 2024 OpenSpec Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Feather badge icons

The README's stars and sponsors badges use the star and heart icons from
Feather v4.29.2, pinned to commit
[`1b002399e8758fb2bccea4d07312dc9e0f43dcb8`](https://github.com/feathericons/feather/tree/1b002399e8758fb2bccea4d07312dc9e0f43dcb8).
The readable sources are retained in
[`docs/assets/feather/star.svg`](docs/assets/feather/star.svg) and
[`docs/assets/feather/heart.svg`](docs/assets/feather/heart.svg), from upstream
[`icons/star.svg`](https://github.com/feathericons/feather/blob/1b002399e8758fb2bccea4d07312dc9e0f43dcb8/icons/star.svg)
and
[`icons/heart.svg`](https://github.com/feathericons/feather/blob/1b002399e8758fb2bccea4d07312dc9e0f43dcb8/icons/heart.svg).

For each badge, the SVG's `fill="none"` is changed to `fill="white"` and
`stroke="currentColor"` to `stroke="white"`, without changing its geometry or
other attributes. The resulting SVG is Base64 encoded as a
`data:image/svg+xml;base64,` URI, then URL encoded for the badge's `logo` query
parameter.

The complete notice from the pinned upstream
[`LICENSE`](https://github.com/feathericons/feather/blob/1b002399e8758fb2bccea4d07312dc9e0f43dcb8/LICENSE)
follows:

```text
The MIT License (MIT)

Copyright (c) 2013-2023 Cole Bemis

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Cargo dependencies and compiled releases

Cargo downloads dependencies separately; their sources are not vendored in this
repository. Each dependency retains its own copyright and license terms. See
`Cargo.lock` for selected versions and the downloaded packages' license files
for their notices. A dependency's SPDX expression (its standardised license
identifier) can contain choices joined by `OR` and simultaneous obligations
joined by `AND`; choosing one branch does not remove other required notices.

Before distributing a compiled Hydragen release, review the dependencies actually
included for that release's targets and features, preserve their required notices,
and meet the GPL's corresponding-source requirements. Adding the project license
and this source-repository notice is not a substitute for that release review.
