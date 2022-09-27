=====
dep2j
=====

.. image:: https://github.com/stnuessl/dep2j/actions/workflows/main.yaml/badge.svg
   :alt: CI
   :target: https://github.com/stnuessl/dep2j/actions

Convert compiler generated dependency files (usually ending in *.d* or *.dep*) 
to JSON.

Table of Contents
=================
.. contents:: \ 

Introduction
============

Given the invocation

.. code::

    dep2j main.d file1.d

**dep2j** will take the input files

.. code::

    # main.d
    main.o: main.c file1.h file2.h

.. code::

   # file1.d
   file1.o: file1.c file1.h

and generate the following output (beautified for readability):

.. code:: json
    
    [
        {
            "target": "main.o",
            "prerequisites": ["main.c", "file1.h", "file2.h"]
        },
        {
            "target": "file1.o",
            "prerequisites": ["file1.c", "file1.h"]
        }
    ]

Motivation
==========

A typical use case within a project is to know about the dependencies
between files. Compilers can emit that information in form of dependency 
(usually .d or .dep) files. However, these file adhere to a Makefile syntax
and I am not aware of any good tools that are able to parse these files.
Also, it seems like there are no other tools which can provide the dependency
information in a suitable machine-readable format. 
The best I've found is 
`clang-scan-deps
<https://github.com/llvm/llvm-project/tree/release/15.x/clang/tools/clang-scan-deps>`_,
but up to at least version 14.0.6 its JSON output is experimental and likely to
be changed in the future.

This project aims to solve this issue by outputting the parsed information
from dependency files in JSON format. Using a well established format
allows to easily build other tools and scripts around the generated output.

Design Goals
============

* Easy program distribution
* Low-latency
* No external dependencies
* Order of input is conserved in generated output
* Automatic merging of prerequisites
* Easy to use
* Minimalistic

Prerequisites
=============

To follow the instructions within this section, the user has to install
the following tools:

* `rust <https://www.rust-lang.org/>`_  >= 1.63
* `make <https://www.gnu.org/software/make/>`_
* `git <https://www.rust-lang.org/>`_ 

Please note that users familiar with **rust** and **cargo** may wish the build 
and install the **dep2j** binary with cargo. If this is the case, it does not
make sense to follow the instructions in the section below.

Installation
============

Execute the commands below to download the project and build the **dep2j** 
binary.

.. code:: sh

   git clone https://github.com/stnuessl/dep2j
   cd dep2j
   make release

Install the resulting dep2j binary to the (default) */usr/local/bin* directory
with

.. code:: sh

   make install

To uninstall **dep2j**, execute as root:

.. code:: sh

   make uninstall

Usage
=====

Synopsis
--------

A generic invocation of **dep2j** is shown below:

.. code:: sh

   dep2j [options] <file0> [... <fileN>]

Examples
--------

Parse dependencies from *file1.d* and *file2.d* and format the generated
output with `python's json.tool
<https://docs.python.org/3/library/json.html#module-json.tool>`_ module.

.. code:: sh

    dep2j file1.d file2.d | python -m json.tool

Retrieve all dependency files from directory *build/*, write their content
to **dep2j**'s standard input, and store the resulting output in *deps.json*.

.. code:: sh

    find build/ -name "*.d" | xargs cat | dep2j -o deps.json

Scan the source code with *clang-scan-deps* and pipe the information to
**dep2j** to print the resulting JSON output to standard output.

.. code:: sh

   clang-scan-deps --compilation-database=<file> | dep2j

Print help message.

.. code:: sh
   
    dep2j --help

Print version information.

.. code:: sh

   dep2j --version

Appendix
========

Tools for beautifing JSON output
--------------------------------

* `json_pp <https://perldoc.perl.org/json_pp>`_
* `json_reformat <http://lloyd.github.io/yajl/>`_ (contained in yajl)
* `json.tool <https://docs.python.org/3/library/json.html#module-json.tool>`_

