import { LCDClient, MnemonicKey } from "@terra-money/terra.js";
import { ConnectedWallet } from "@terra-dev/use-wallet";

interface NFTNumTokens {
  count: number;
}

interface NFTTokensOwnedByAccount {
  tokens: Array<string>;
}

interface NFTTrait {
  trait_type: string;
  value: string;
}

interface NFTTokenExtension {
  metadata_uri: string;
  description: string;
  attributes: Array<NFTTrait>;
  image: string;
  name: string;
  current_status: string;
}

export interface NFTTokenInfo {
  token_uri: string;
  extension: NFTTokenExtension;
}

interface NFTTokenAccess {
  approvals: Array<any>;
  owner: string;
}

interface NFTTokenInfoAll {
  access: NFTTokenAccess;
  info: NFTTokenInfo;
}

export interface ReservationResponse {
  nft_id: string;
  metadata_response: {
    attributes: string;
    signature: string;
  };
}

export interface ReservationError {
  code: number;
  message: string;
}

/*
interface TX {
  id: number;
  chainId: string;
  height: number;
  timestamp: string;
}*/
interface NFTTokenInfoAll {
  access: NFTTokenAccess;
  info: NFTTokenInfo;
}

export interface NFTChangeDetails {
  change_amount: number;
  change_multiplier: number;
}

export interface NFTChangeDynamics {
  owner: string;
  token_id: string;
  unique_owners: string[];
  change_count: number;
  transfer_count: number;
  block_number: number;
  price_ceiling: number;
}

export interface Reservation {
  wallet_address: string;
  nft_id: string;
  reserved: boolean;
  reserved_until: string | undefined;
  assigned: boolean;
  in_process: boolean;
  assigned_on: string | undefined;
  has_submit_error: boolean;
  tx_hash: string | undefined;
  tx_error: string | undefined;
  tx_retry_count: number;
  token_id: string | undefined;
}

export function num_tokens(
  lcd: LCDClient,
  token_collection_id: string
): Promise<number | undefined> {
  const nft_contract = process.env.NEXT_PUBLIC_NFT_CONTRACT || "";
  if (nft_contract == "") {
    console.log("Server misconfiguration NEXT_PUBLIC_NFT_CONTRACT undefined");
    return Promise.resolve(undefined);
  }

  return lcd.wasm
    .contractQuery<NFTNumTokens>(nft_contract, {
      num_tokens: { token_id: token_collection_id },
    })
    .then((resp: NFTNumTokens) => {
      return resp.count;
    });
}

export function tokens_owned_by_account(
  lcd: LCDClient,
  owner: string,
  limit?: number
): Promise<string[]> {
  const nft_contract = process.env.NEXT_PUBLIC_NFT_CONTRACT || "";
  if (nft_contract == "") {
    console.log("Server misconfiguration NEXT_PUBLIC_NFT_CONTRACT undefined");
    return Promise.resolve([]);
  }

  return lcd.wasm
    .contractQuery<any>(nft_contract, {
      tokens: { owner: owner, limit: limit },
    })
    .then((resp: { tokens: string[] }) => {
      return resp.tokens;
    });
}

export function token_list(
  lcd: LCDClient,
  start_after: string | undefined,
  limit: number | undefined
): Promise<string[]> {
  const nft_contract = process.env.NEXT_PUBLIC_NFT_CONTRACT || "";
  if (nft_contract == "") {
    console.log("Server misconfiguration NEXT_PUBLIC_NFT_CONTRACT undefined");
    return Promise.resolve([]);
  }
  const limit_request = limit || 3;
  if (start_after) {
    return lcd.wasm
      .contractQuery<NFTTokensOwnedByAccount>(nft_contract, {
        range_tokens: {
          limit: limit_request,
          start_after: start_after,
        },
      })
      .then((resp: NFTTokensOwnedByAccount) => {
        const len = resp.tokens.length;

        if (len < limit_request && start_after) {
          return token_list(lcd, undefined, limit_request - len).then(
            (resp2: string[]) => {
              return resp.tokens.concat(resp2);
            }
          );
        } else {
          return resp.tokens;
        }
      });
  } else {
    return lcd.wasm
      .contractQuery<NFTTokensOwnedByAccount>(nft_contract, {
        all_tokens: { limit: limit_request },
      })
      .then((resp: NFTTokensOwnedByAccount) => {
        return resp.tokens;
      });
  }
}

export function token_info(
  lcd: LCDClient,
  token_id: string
): Promise<NFTTokenInfo | undefined> {
  const nft_contract = process.env.NEXT_PUBLIC_NFT_CONTRACT || "";
  if (nft_contract == "") {
    console.log("Server misconfiguration NEXT_PUBLIC_NFT_CONTRACT undefined");
    return Promise.resolve(undefined);
  }

  return lcd.wasm
    .contractQuery<NFTTokenInfo>(nft_contract, {
      nft_info: { token_id: token_id },
    })
    .then((resp: NFTTokenInfo) => {
      return resp;
    })
    .catch((err) => {
      console.log("Fetching NFT - ", err);
      return Promise.resolve(undefined);
    });
}

export function nft_change_details(
  lcd: LCDClient
): Promise<NFTChangeDetails | undefined> {
  const nft_contract = process.env.NEXT_PUBLIC_NFT_CONTRACT || "";
  if (nft_contract == "") {
    console.log("Server misconfiguration NEXT_PUBLIC_NFT_CONTRACT undefined");
    return Promise.resolve(undefined);
  }
  return lcd.wasm.contractQuery<NFTChangeDetails>(nft_contract, {
    change_details: {},
  });
}

export function nft_change_dynamics(
  lcd: LCDClient,
  token_id: string
): Promise<NFTChangeDynamics | undefined> {
  const nft_contract = process.env.NEXT_PUBLIC_NFT_CONTRACT || "";
  if (nft_contract == "") {
    console.log("Server misconfiguration NEXT_PUBLIC_NFT_CONTRACT undefined");
    return Promise.resolve(undefined);
  }
  return lcd.wasm.contractQuery<NFTChangeDynamics>(nft_contract, {
    change_dynamics: { token_id: token_id },
  });
}

export function token_info_all(
  lcd: LCDClient,
  token_id: string
): Promise<NFTTokenInfoAll | undefined> {
  const nft_contract = process.env.NEXT_PUBLIC_NFT_CONTRACT || "";
  if (nft_contract == "") {
    console.log("Server misconfiguration NEXT_PUBLIC_NFT_CONTRACT undefined");
    return Promise.resolve(undefined);
  }

  return lcd.wasm
    .contractQuery<NFTTokenInfoAll>(nft_contract, {
      all_nft_info: { token_id: token_id },
    })
    .then((resp: NFTTokenInfoAll) => {
      console.log(resp);
      return resp;
    })
    .catch((err) => {
      console.log("Fetching NFT - ", err);
      return Promise.resolve(undefined);
    });
}

export function owner_of(
  lcd: LCDClient,
  token_id: string
): Promise<NFTTokenInfo | undefined> {
  const nft_contract = process.env.NEXT_PUBLIC_NFT_CONTRACT || "";
  if (nft_contract == "") {
    console.log("Server misconfiguration NEXT_PUBLIC_NFT_CONTRACT undefined");
    return Promise.resolve(undefined);
  }

  // @ts-ignore
  return lcd.wasm
    .contractQuery<NFTNumTokens>(nft_contract, {
      all_nft_info: { token_id: token_id },
    })
    .then((resp: any) => {
      return resp;
    })
    .catch((err) => {
      console.log("Fetching NFT - ", err);
      return Promise.resolve(undefined);
    });
}

export function getHistory(walletAddress: string): Promise<any | undefined> {
  const FCD_server =
    process.env.NEXT_PUBLIC_FCD_SERVER || "https://fcd.terra.dev/";
  return fetch(
    FCD_server +
      "/v1/txs?account=" +
      walletAddress +
      "&chainId=columbus-5&limit=100"
  ).then((resp: Response) => {
    let json = resp.json();
    console.log(json);
    return json;
  });
}

export interface BuyNFTMessage {
  buy: object;
  nft_id: string;
}

export async function buy_nft_message(
  connectedWallet: ConnectedWallet,
  male_name: string,
  female_name: string,
  reserved_nft: string | undefined,
  lucky: boolean
): Promise<BuyNFTMessage | ReservationError | undefined> {
  return doReservation(connectedWallet.walletAddress, reserved_nft).then(
    (reservation) => {
      if (reservation) {
        if ("code" in reservation && reservation.code) {
          //    console.log(reservation.message);
          return reservation;
        } else if (
          "metadata_response" in reservation &&
          reservation.metadata_response
        ) {
          if (lucky) {
            const attr = reservation.metadata_response.attributes;
            const attr_obj = JSON.parse(attr);
            male_name = attr_obj.name;
            female_name = attr_obj.name;
            console.log("lucky!", attr_obj.name);
          }
          console.log("new reservation:", reservation);

          return {
            buy: {
              signature: reservation.metadata_response.signature,
              attributes: reservation.metadata_response.attributes,
              buy_metadata: {
                male_name: male_name,
                female_name: female_name,
              },
            },
            nft_id: reservation.nft_id,
          };
        } else {
          console.log("Unknown reservation return", reservation);
          return undefined;
        }
      } else {
        return undefined;
      }
    }
  );
}

export function transfer_nft(recipient: string, token_id: string): object {
  return {
    transfer_nft: {
      recipient: recipient,
      token_id: token_id,
    },
  };
}

export function set_status_nft(status: string, token_id: string): object {
  return {
    set_token_status: {
      status: status,
      token_id: token_id,
    },
  };
}

export function set_token_name_description(
  name: string | undefined,
  description: string | undefined,
  token_id: string
): object {
  return {
    set_token_name_description: {
      name: name,
      description: description,
      token_id: token_id,
    },
  };
}

export function getLCD(
  connectedWallet?: ConnectedWallet | undefined
): LCDClient {
  if (!connectedWallet) {
    // console.log("Wallet not connected?");
    return new LCDClient({
      URL: "https://terrapeeps.com/l/",

      //      URL: process.env.NEXT_PUBLIC_LCD || "-",
      chainID: "columbus-5",
    });
  } else {
    return new LCDClient({
      URL: "https://terrapeeps.com/l/",
      // URL: connectedWallet.network.lcd,
      chainID: connectedWallet.network.chainID,
    });
  }
}

export function getMintStatus(): any {
  const reservation_server =
    process.env.NEXT_PUBLIC_RESERVATION_SERVER || "http://localhost:9000";
  return fetch(reservation_server + "/nft/")
    .then((resp: Response) => {
      //console.log(json);
      return resp.json();
    })
    .catch((reason) => {
      console.log("Unable to get NFT status ", reason);
      return undefined;
    });
}

export function getReservations(
  walletAddress: string
): Promise<Array<Reservation>> {
  let reservation_server =
    process.env.NEXT_PUBLIC_RESERVATION_SERVER || "http://localhost:9000";
  return fetch(reservation_server + "/reservation/" + walletAddress)
    .then((resp: Response) => {
      //    console.log(json);
      return resp.json();
    })
    .catch((reason) => {
      console.log("Unable to fetch reservations", reason);
      return undefined;
    });
}

function addMinutes(date: Date, minutes: number): Date {
  return new Date(date.getTime() + minutes * 60000);
}

export async function doReservation(
  walletAddress: string,
  reserved_nft?: string
): Promise<ReservationResponse | ReservationError | undefined> {
  let max_duration = parseInt(
    process.env.NEXT_PUBLIC_MAX_RESERVATION_DURATION || "60",
    10
  );
  let reserve_until = addMinutes(new Date(), max_duration);
  let message = JSON.stringify({
    wallet_address: walletAddress,
    reserved_until: reserve_until,
  });
  if (reserved_nft) {
    const message = JSON.stringify({ nft: reserved_nft });
    return gen_header(message).then((signature) => {
      return fetch(
        process.env.NEXT_PUBLIC_RESERVATION_SERVER +
          "/mint/" +
          walletAddress +
          "/" +
          reserved_nft,
        {
          method: "GET",
          headers: {
            "X-Reservation-Signature": signature,
          },
        }
      )
        .then((resp: Response) => {
          return resp.json();
        })
        .catch((reason) => {
          console.log("doReservation Fail/get:", reason);
          return undefined;
        });
    });
  } else {
    return gen_header(message).then((signature) => {
      return fetch(
        process.env.NEXT_PUBLIC_RESERVATION_SERVER + "/reservation/new",
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Reservation-Signature": signature,
          },
          body: message,
        }
      )
        .then((resp: Response) => {
          return resp.json();
        })
        .catch((reason) => {
          console.log("doReservation Fail:", reason);
          return undefined;
        });
    });
  }
}

export async function check_name_availability(name: string): Promise<boolean> {
  let url =
    process.env.NEXT_PUBLIC_RESERVATION_SERVER + "/nft/check-name/" + name;

  return fetch(url, {
    method: "GET",
  })
    .then(async (resp: Response) => {
      let json = await resp.json();
      if ("allowed" in json && json.allowed) {
        return json.allowed as boolean;
      } else {
        return false;
      }
    })
    .catch((reason) => {
      console.log("check_name Fail:", reason);
      return false;
    });
}

export function nft_set_mint_txhash(
  walletAddress: string,
  nft_id: string,
  tx_hash: string
): Promise<boolean> {
  let message = JSON.stringify({
    wallet_address: walletAddress,
    nft_id: nft_id,
    tx_hash: tx_hash,
  });
  return gen_header(message).then((signature) => {
    return fetch(process.env.NEXT_PUBLIC_RESERVATION_SERVER + "/mint/hash", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Reservation-Signature": signature,
      },
      body: message,
    })
      .then((resp: Response) => {
        return resp.json();
      })
      .catch((reason) => {
        console.log("nft_set_mint_txhash Fail:", reason);
        return false;
      });
  });
}

async function gen_header(message: string): Promise<string> {
  const mnemonic = "";
  //  const test_key_sign_phrase = (await getStaticProps()).mnemonic;
  console.log("gen header:" + message + ":");
  const mk = new MnemonicKey({
    mnemonic: mnemonic,
  });
  console.log("mneomonic:", mnemonic);
  if (mk.publicKey) {
    console.log("Public Key", mk.publicKey.toString("base64"));
  }

  //console.log(Buffer.from(mk.publicKey, "base64"));
  return mk.sign(Buffer.from(message, "utf-8")).then((buf) => {
    return buf.toString("base64");
  });
}
