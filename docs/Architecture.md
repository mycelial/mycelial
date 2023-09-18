# Mycelial Architecture

Mycelial allows you to easily move data from sources, that are usually at the
edge of your network, to destinations where that information can be further used
and analyzed.

## So how does Mycelial work?

A Mycelial application is made up of one server and potentially many clients.
Each client is typically installed on edge devices or other nodes and configured
via a local [config.toml](Client_Configuration.md) file.  The configuration file
specifies the client's information such as its Name, the associated server
address, as well as its data sources and/or destinations.

When a client is started, it's responsible for connecting to the server and
registering its sources and destinations with the server.

After the client has registered its sources and destinations with the server,
then the server can be used to connect the sources and destinations between one
or more clients into what's referred to as a data pipeline specifications. 

Data pipelines specifications represent the work or job that needs to be
performed to move your important information from your edge devices to your
destination systems, where that data can be further used and analyzed.

Once you've created and published a data pipeline on the server via the UI or
via API calls, the data pipeline will get downloaded by the relevant client(s)
and executed.

## What makes up a data pipeline specification?

Pipelines are made up of two or more sections. Each section implements a trait
in Rust, which are like protocols or interfaces in other languages. Each section
will have Inputs, Outputs and a code execution entry point. Note that source
sections don't receive inputs, they generate their own data which is returned
via its output. Additionally, destination sections don't return outputs, they
persist the inputs in the destination system. Pipelines are executed via a 
scheduler on the associated client(s).

Each section within a pipeline communicates with subsequent sections via a 
unified message format, which is an arrow dataframe wrapped in some metadata.