# buildigvm
A tool to build an IGVM file that contains an OVMF firmware image that can be
deployed to any hypervisor that provides a compatible IGVM loader.

The IGVM files generated with this tool contain directives that populate the
provided OVMF firmware into initial guest memory state aligning the top of the
firmware to 4GB. Depending on the isolation platform supported by the host, the
builder will also generate an initial CPU state for each virtual processor.

A hypervisor that supports IGVM will obey these directives and populate the
firmware into guest memory, and setup the initial CPU states prior to launching
the guest. For SEV platforms, the firmware pages and initial state will be
measured and result in a startup image that can be verified via remote
attestation.

## CPU count
For platforms that support protected CPU state, such as AMD SEV-ES and AMD
SEV-SNP, `buildigvm` needs to know how many virtual processors to create
directives for in the IGVM file. This can be provided via the `--cpucount`
parameter and must be set to a value greater than or equal to the number of CPUs
allocated to the guest in the hypervisor.

## Usage
`Usage: buildigvm [OPTIONS] --firmware <FIRMWARE> --output <OUTPUT> --cpucount <CPUCOUNT> <PLATFORM>`

### Arguments:
```
  <PLATFORM>
          Possible values:
          - sev:     AMD SEV
          - sev-es:  AMD SEV-ES
          - sev-snp: AMD SEV-SNP
          - native:  An X86-64 platform that does not include support for any isolation technology
```

### Options:
```
  -f, --firmware <FIRMWARE>
          Firmware file, e.g. OVMF.fd

  -o, --output <OUTPUT>
          Output filename for the generated IGVM file

  -c, --cpucount <CPUCOUNT>
          

  -v, --verbose
          Print verbose output

  -h, --help
          Print help (see a summary with '-h')
```

## Examples
### AMD SEV
This command line generates an IGVM file that is compatible with AMD SEV guests
with up to 4 virtual CPUs:

```bash
$ buildigvm --firmware $OVMF_PATH/OVMF.fd --output sev.igvm --cpucount 4 sev
```

### AMD SEV-ES
This command line generates an IGVM file that is compatible with AMD SEV-ES
guests with up to 4 virtual CPUs:

```bash
$ buildigvm --firmware $OVMF_PATH/OVMF.fd --output sev-es.igvm --cpucount 4 sev-es
```

### AMD SEV-SNP
This command line generates an IGVM file that is compatible with AMD SEV-SNP
guests with up to 4 virtual CPUs:

```bash
$ buildigvm --firmware $OVMF_PATH/OVMF.fd --output sev-snp.igvm --cpucount 4 sev-snp
```
