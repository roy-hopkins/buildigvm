# buildigvm
A tool to build an IGVM file that contains an OVMF firmware image that can be
deployed to any hypervisor that provides a compatible IGVM loader.

The IGVM files generated with this tool contain directives that populate the provided
OVMF firmware into initial guest memory state aligning the top of the firmware
to 4GB. Depending on the isolation platform supported by the host, the builder
will also generate an initial CPU state for each virtual processor.

A hypervisor that supports IGVM will obey these directives and populate the
firmware into guest memory, and setup the initial CPU states prior to launching
the guest. It will also instruct the SEV PSP to measure the firmware pages and
state resulting in a startup image that can be verified via remote attestation.

## CPU count
For platforms that support protected CPU state, such as AMD SEV-ES and AMD
SEV-SNP, the tool needs to know how many virtual processors to create directives
for in the IGVM file. This can be provided via the `--cpucount` parameter and
must be set to a value greater than or equal to the number of CPUs allocated to
the guest in the hypervisor.

## Supported Isolation Platforms
Currently `buildigvm` can create IGVM files that can be used to launch guests
that support either AMD SEV, AMD SEV-ES or AMD SEV-SNP (using the same IGVM file
as for SEV-ES).

## Usage
`Usage: buildigvm [OPTIONS] --firmware <FIRMWARE> --output <OUTPUT> --cpucount <CPUCOUNT> <PLATFORM>`

### Arguments:
```
  <PLATFORM>
          Possible values:
          - sev:    Build an IGVM file compatible with SEV
          - sev-es: Build an IGVM file compatible with SEV-ES
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

## Example usage for AMD SEV
This command line generates an IGVM file that is compatible with AMD SEV guests
with up to 4 virtual CPUs:

```bash
$ buildigvm --firmware $OVMF_PATH/OVMF.fd --output sev.igvm --cpucount 4 sev
```

## Example usage for AMD SEV-ES
This command line generates an IGVM file that is compatible with AMD SEV-ES
guests with up to 4 virtual CPUs:

```bash
$ buildigvm --firmware $OVMF_PATH/OVMF.fd --output sev-es.igvm --cpucount 4 sev-es
```
