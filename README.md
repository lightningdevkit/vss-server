# Versioned Storage Service

### Introduction to VSS (Versioned Storage Service)

VSS, which stands for Versioned Storage Service, is an open-source project designed to offer a server-side cloud storage
solution specifically tailored for non-custodial Lightning supporting mobile wallets. Its primary objective is to
simplify the development process for Lightning wallets by providing a secure means to store and manage the essential
state required for Lightning Network (LN) operations.

In a non-custodial Lightning wallet, it is crucial to securely store and manage various types of state data. This
includes maintaining a list of open channels with other nodes in the network and updating the channel state for every
payment made or received. Relying solely on user devices to store this information is not reliable, as data loss could
lead to the loss of funds or even the entire wallet.

To address this challenge, the VSS project introduces a framework and a readily available service that can be hosted by
anyone as a Versioned Storage Service. It offers two core functionalities:

* **Recovery**: In the event of a user losing their phone or access to their app's data, VSS allows for the restoration
  of the wallet state. This ensures that users can regain access to their funds, even in cases of device or data loss.
* **Multi-device Access**: VSS enables multiple devices with the same wallet app to securely access and share LN state.
  This seamless switching between devices ensures consistent access to funds for users.

<p align="center">
  <img src="http://www.plantuml.com/plantuml/png/VP2nJWCn44HxVyMKK4JqAQ8W8aGHA33GBxuXP-7p7lRUeVmzAz60X6YcsQTvezrtasRBL89bAyHBZBZBfn57hYmuY0bkYtw6SA-lkV30DITkTd1mY-l5HbRBIInhnIC_5dOBVjliVl9RT9ru8Ou_wJlhPGX5TSQRDhYddJ7BUV8cT8-hniIySlZJ-JmFOiJn0JUZrCg2Q6BybaRJ9YVwCjCff_zWUE7lZN59YRq7rY7iFVmhNm00" />
</p>

Clients can also use VSS for general metadata storage as well such as payment history, user metadata etc.

### Motivation

By providing a reusable component, VSS aims to lower the barriers for building high-quality LN wallets. Wallet
developers have the flexibility to either host the VSS service in-house, enabling easy interaction with the component,
or utilize reliable third-party VSS providers if available.

VSS is designed to work with various applications that implement different levels of key-level versioning and
data-integrity mechanisms. It even allows for the disabling of versioning altogether for single-device wallet usage,
making it simple to get started.

The project's design decisions prioritize features such as multi-device access, user privacy through client-side
encryption(e.g. using key derived from bitcoin wallet), authorization mechanisms, data and version number verifiability,
and modularity for seamless integration with different backend technologies.

### Modularity

VSS can work out-of-box with minor configuration but is intended to be forked and customized based on the specific needs
of wallet developers. This customization may include implementing custom authorization, encryption, or backend database
integration with different cloud providers. As long as the API contract is implemented correctly, wallets can
effortlessly switch between different instances of VSS.

VSS ships with a PostgreSQL implementation by default and can be hosted in your favorite infrastructure/cloud provider (
AWS/GCP) and its backend storage can be switched with some other implementation for KeyValueStore if needed.

### Project Status

VSS execution is split into two phases: Phase I prioritizes recovery and single-device use, whereas Phase II covers
multi-device use. Phase I is ready to use and integrated within [LDK-node]. Phase II will be subject to monitoring for
demand from wallets and may slip to a later date. Refer to [LDK-Roadmap] for more details.

### Summary

In summary, VSS is an open-source project that offers a server-side cloud storage solution for non-custodial Lightning
wallets. It provides multi-device access, recovery capabilities, and various features to ensure user privacy and data
verifiability. By leveraging VSS, wallet developers can focus on building innovative Lightning wallets without the
burden of implementing complex storage solutions from scratch.

[LDK-node]: https://github.com/lightningdevkit/ldk-node

[LDK-Roadmap]: https://lightningdevkit.org/blog/ldk-roadmap/#vss
