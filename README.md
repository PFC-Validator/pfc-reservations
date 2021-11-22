# PFC NFT Reservation system

## TL;DR
This service allows a NFT minter to upload their minting details in a secure fashion to a centralized db.
users can then 'reserve' a NFT in a deterministically pseudo-random way (seed is based on wallet address), 

once reserved, we either returned a signed version of the meta-data which can be used to:
- execute the mint directly by the end user.
- allow the end user to sign the overall transaction, with the service submitting the transaction via their own private LCD
- check for payment by some other means and issue the mint transaction by the NFT contract owner (the traditional method)

*NOTE* currently not deterministically pseudo-random.

It serves [TerraPeeps](https://terrapeeps.com) needs. It may serve yours.

If you think this was useful, feel free to delegate to the [PFC](https://station.terra.money/validator/terravaloper12g4nkvsjjnl0t7fvq3hdcw7y8dc9fq69nyeu9q) validator. It will help defray the costs.

[PFC](https://twitter.com/PFC_Validator) - As Terra is all about Pursuing Flights of Charm right... feel free to drop me a line


## todo
- stage-close .
- two level signature verification. (admin functions require a different signature than the user-facing 'reservation' functions)

# Typescript
This is  [JS](https://github.com/PFC-Validator/pfc-reservations/blob/main/js/nft.ts) I use to interact with the reservation server, and the Terra blockchain.

I use next, but it should work elsewhere.
I have the following in my '.env' file:
```shell
NEXT_PUBLIC_NFT_CONTRACT=
NEXT_PUBLIC_CHAIN=bombay-12
NEXT_PUBLIC_LCD=https://bombay-lcd.terra.dev
NEXT_PUBLIC_FCD=https://bombay-fcd.terra.dev
NEXT_PUBLIC_RESERVATION_SERVER=http://localhost:8000
NEXT_PUBLIC_MAX_RESERVATION_DURATION=120
NEXT_PUBLIC_TX_URL=https://finder.terra.money/bombay-12/tx/
```

## WARNING
there is currently a security concern/issue with the typescript. as everything is 'client side', the mnemonic used to generated signatures
is visible.. what this means is that people can use the 'reservation' calls outside of the web app. 
The smart contract still validates signatures (with a different key)

There is a TODO above to help with this... feel free to submit a PR