# Hydragen glossary

This glossary defines Hydragen's terms using their agreed meaning.
[FOUNDATIONS.md](FOUNDATIONS.md) describes the product's goals and guiding values.

## Layer

Hydragen's core building block. A layer can have no [dependencies](#dependency) or
depend on one or more other layers.

Both [root layers](#root-layer) and [dependent layers](#dependent-layer) can have
other layers depend on them. Layers can form [dependency](#dependency) chains of
any depth; there is no fixed maximum number of levels.

Every layer has a [manifest](#manifest) at its repository root, declaring its
required name, optional author and description, and the other layers it depends
on. Its filename is one of `layer.yaml`, `layer.yml`, `layer.toml`, `layer.json`,
or `layer.jsonc`.

## Root layer

A [layer](#layer) with no [dependencies](#dependency). Its
[manifest](#manifest) may omit the [dependency](#dependency) list or include an
empty list; both mean the same thing.

Its file [contributions](#contribution) are [additions](#addition). With no
[inherited content](#inherited-content), it cannot have
[modification](#modification) or [deletion](#deletion)
[contributions](#contribution).

## Dependent layer

A [layer](#layer) that depends on one or more other [layers](#layer). Each
[dependency](#dependency) may itself be a [root layer](#root-layer) or another
dependent layer.

It commits its [manifest](#manifest) and only the content needed to describe its
own [contributions](#contribution): [additions](#addition),
[modifications](#modification), and [deletions](#deletion) relative to its
[dependencies](#dependency).
Unchanged [inherited content](#inherited-content) is not committed again in the
dependent layer.

Developers and agents edit its [layer result](#layer-result) in its
[workspace](#workspace). The [Hydragen CLI](#hydragen-cli) translates between those
editable files and its recorded [contributions](#contribution).

## Dependency

A [layer](#layer) that another [layer](#layer) declares in its [manifest](#manifest)
as a source of [inherited content](#inherited-content). Its
[layer result](#layer-result) provides that content to the
[dependent layer](#dependent-layer).

Being a dependency is a role relative to another [layer](#layer), not a separate
kind of [layer](#layer). Both [root layers](#root-layer) and
[dependent layers](#dependent-layer) can serve as dependencies.

## Inherited content

Content supplied by a [layer's](#layer) [dependencies](#dependency) through their
[layer results](#layer-result), before that [layer's](#layer) own
[contributions](#contribution) are applied.

## Reuse relationship

The ongoing connection between a [dependent layer](#dependent-layer) and its
[dependencies](#dependency), including their selected versions, that allows it to
keep benefiting from shared improvements while preserving its own
[contributions](#contribution).

## Layer result

The files and directory structure produced by applying a [layer's](#layer)
[contributions](#contribution) to its [inherited content](#inherited-content).
Unchanged inherited files and [additions](#addition) are present as ordinary
project files, [modifications](#modification) appear in the resulting file
contents rather than as [patches](#patch), and [deletions](#deletion) appear as
absent files rather than as [deletion markers](#deletion-marker).

## Workspace

The directory in which the [Hydragen CLI](#hydragen-cli)
[renders](#rendering) a [dependent layer's](#dependent-layer)
[layer result](#layer-result) and developers or agents edit it.

Developers and agents work directly on these files without having to express each
edit as a [contribution](#contribution) themselves. The files in the workspace
are not committed. The [Hydragen CLI](#hydragen-cli) uses
[contribution capture](#contribution-capture) to reflect edits there in the
[contributions](#contribution) that are committed instead.

## Rendering

The process by which the [Hydragen CLI](#hydragen-cli) produces a
[layer result](#layer-result) in a [workspace](#workspace) from the
[layer's](#layer) [contributions](#contribution) and its [dependencies](#dependency).

## Contribution capture

The process by which the [Hydragen CLI](#hydragen-cli) translates edits in a
[dependent layer's](#dependent-layer) [workspace](#workspace) into the files that
represent its current [contributions](#contribution) for committing. This prepares
the [contribution](#contribution) files; it does not make a Git commit or record a
running history of edits in their contents.

## Hydragen CLI

Hydragen's command-line interface: the tool developers and agents run from a
terminal. It handles translation between a [dependent layer's](#dependent-layer)
[workspace](#workspace) and its [contributions](#contribution),
[rendering](#rendering) the [layer result](#layer-result) for editing and using
[contribution capture](#contribution-capture) to reflect edits in the files that
represent those [contributions](#contribution) for committing.

## Manifest

A file at the root of a [layer's](#layer) repository, named exactly `layer.yaml`,
`layer.yml`, `layer.toml`, `layer.json`, or `layer.jsonc`. The two YAML extensions
use the same format; the others use TOML, strict JSON, and JSON with comments
(JSONC), respectively. New manifests default to `layer.yaml`. The
[layer's](#layer) identifying name does not change the filename.

It declares the [layer's](#layer) required name and optional author and description,
and lists its [dependencies](#dependency). Each supplied metadata field must be
a non-blank string, with the same meaning in every supported format.

If multiple supported names are occupied, selection follows the fixed order
`layer.yaml` > `layer.yml` > `layer.toml` > `layer.json` > `layer.jsonc`. Only the
selected file supplies data; manifests are not merged, and a selected-file error
does not cause fallback to another format.

## Contribution

What a [layer](#layer) supplies itself rather than inherits: [additions](#addition)
of files, [modifications](#modification) to inherited files, or
[deletions](#deletion) of inherited files.

A [layer's](#layer) committed files describe its current contributions through
their contents or their presence, including its differences from
[dependencies](#dependency).
They are not a running history of edits over time. Git can track how those
committed files evolve; Hydragen does not require a separate edit history in
their contents.

For example, editing or removing a [root layer's](#root-layer) own file does not
create a [modification](#modification) or [deletion](#deletion) of
[inherited content](#inherited-content).

[Additions](#addition) are committed as ordinary files,
[modifications](#modification) use [patches](#patch), and [deletions](#deletion) use
[deletion markers](#deletion-marker).

For a [dependent layer](#dependent-layer), these committed representations are
distinct from the [layer result](#layer-result) edited in its
[workspace](#workspace).
The [Hydragen CLI](#hydragen-cli) handles translation between the two.

## Contribution type

The category describing what a [contribution](#contribution) does: an
**[addition](#addition)**, **[modification](#modification)**, or
**[deletion](#deletion)**. "Change" is a general word, not a formal type.

## Addition

A [contribution](#contribution) that introduces a file not inherited from
[dependencies](#dependency). Both [root layers](#root-layer) and
[dependent layers](#dependent-layer) can have additions.

The file itself is committed using its ordinary name and native format. An
addition needs no special filename or Hydragen-specific file format.

## Modification

A [contribution](#contribution) that alters an inherited file while keeping that
file in the [layer result](#layer-result). Only
[dependent layers](#dependent-layer) can have modifications, because
[root layers](#root-layer) have no inherited files to modify.

A modification uses a [patch](#patch) file. Removing a function, setting, or
paragraph from an inherited file is a modification, not a [deletion](#deletion):
the file itself remains in the [layer result](#layer-result).

## Patch

A file used by a [modification](#modification) [contribution](#contribution) to
describe how an inherited file is altered. Its filename is the original file's
name with `.patch` appended. For example, a [modification](#modification) to
`package.json` uses a file named `package.json.patch`.

## Deletion

A [contribution](#contribution) that explicitly removes an inherited file from a
[layer result](#layer-result). Only [dependent layers](#dependent-layer) can have
deletions, because [root layers](#root-layer) have no
[inherited content](#inherited-content) to remove.

A deletion is recorded by a [deletion marker](#deletion-marker).

Simply omitting [inherited content](#inherited-content) from the [layer's](#layer)
repository is not a deletion: unchanged [inherited content](#inherited-content) is
already omitted from a [dependent layer's](#dependent-layer) commits.

## Deletion marker

An empty file used by a [deletion](#deletion) [contribution](#contribution) to
identify the inherited file to remove. Its filename is the original file's name
with `.delete` appended. For example, `README.md.delete` records the
[deletion](#deletion) of inherited `README.md`.

The marker must have no content. Its presence alone records the
[layer's](#layer) [deletion](#deletion) of that file.

## Authoritative source

The maintained definition that governs a piece of shared content and where
improvements to that shared definition belong.

Authority can apply to part of a file rather than the whole file: different
[layers](#layer) can supply different parts through their
[contributions](#contribution).

## Drift

The gradual divergence of separately maintained versions of code, configuration,
or other project files. For example, projects that start with copies of the same
configuration can develop different versions as each copy is edited separately.
