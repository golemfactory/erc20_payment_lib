import BackendSettings from "./BackendSettings";

interface BackendFetch {
    method?: string;
    body?: string;
    headers?: Headers;
}

export function backendFetch(backendSettings: BackendSettings, uri: string, params?: BackendFetch): Promise<Response> {
    const headers = params?.headers ?? new Headers();
    const method = params?.method ?? "GET";
    const body = params?.body;

    let url = uri;
    if (uri.startsWith("/")) {
        if (backendSettings.backendUrl.endsWith("/")) {
            url = backendSettings.backendUrl + uri.substring(1);
        } else {
            url = backendSettings.backendUrl + uri;
        }
    } else {
        throw new Error("Uri must start with /");
    }

    if (backendSettings.enableBearerToken) {
        headers.append("Authorization", "Bearer " + backendSettings.bearerToken);
    }
    if (body) {
        headers.append("Content-Type", "application/json");
    }
    console.log("Calling backend: " + url);

    return fetch(url, {
        method: method,
        headers: headers,
        body: body,
    });
}
