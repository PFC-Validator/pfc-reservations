# PFC NFT Reservation system

## TL;DR
This service allows a NFT minter to upload their minting details in a secure fashion to a centralized db.
users can then 'reserve' a NFT in a deterministically pseudo-random way (seed is based on wallet address), 

once reserved, we either returned a signed version of the meta-data which can be used to:
- execute the mint directly by the end user.
- allow the end user to sign the overall transaction, with the service submitting the transaction via their own private LCD
- check for payment by some other means and issue the mint transaction by the NFT contract owner (the traditional method)

*NOTE* currently not deterministically pseudo-random.