
# Enabling Federated Learning at the Edge through the IOTA Tangle
### ***Abstract***
The proliferation of Internet of Things (IoT) devices, generating massive amounts of heterogeneous distributed data, has pushed toward edge cloud computing as a promising paradigm to bring cloud capabilities closer to data sources. In many cases of practical interest, centralized Machine Learning (ML) approaches can hardly be employed due to high communication costs, low reliability, legal restrictions, and scalability issues. Therefore, Federated Learning (FL) is emerging as a promising distributed ML approach that enables models to be trained on remote devices using their local data. However, "traditional" FL solutions still present open technical challenges, such as single points of failure and lack of trustworthiness among participants. To address these open challenges, some researchers have started to propose leveraging blockchain technologies. However, the adoption of blockchain for FL at the edge is limited by several factors nowadays, such as long waiting times for transaction confirmation and high energy consumption. In this work, we conduct an original and comprehensive analysis of the key design challenges to address towards an efficient implementation of FL at the edge, and analyze how Distributed Ledger Technologies (DLTs) can be employed to overcome them. Then, we present a novel architecture that enables FL at the edge by leveraging the IOTA Tangle, a next-generation DLT whose data structure is a directed acyclic graph (DAG), and the InterPlanetary File System (IPFS) to store and share partial models. Experimental results demonstrate the feasibility and efficiency of our proposed solution in real-world deployment scenarios
## Create the Containers
Launch the following commands in the respective directories:

    docker build -t as-image .
    docker build -t client-image .
    docker build -t client-python-image .

## Deployment
Deploy a private [_one-click-tangle_](https://github.com/iotaledger/one-click-tangle) instance and an [_IPFS cluster_](https://github.com/pccr10001/ipfs-multinode-cluster). After doing so, it is possible to launch the various containers of the components. From the respective directories:

    docker run -i --name="as" -v $(pwd)/src:/mnt --network="host" as-image
    docker run -i -v $(pwd)/src/clientN:/mnt --network="host" --name="clientN" -e PORT="555N" -e CLIENTS="Number of clients" client-image
    docker run -i -v $(pwd)/src/clientN:/mnt --network="host" --name="client-pythonN" -e PORT="555N" -e CLIENTS="Number of clients" client-python-image
    

    



