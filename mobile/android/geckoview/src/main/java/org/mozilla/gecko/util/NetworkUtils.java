/* -*- Mode: Java; c-basic-offset: 4; tab-width: 20; indent-tabs-mode: nil; -*-
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.gecko.util;

import android.content.Context;
import android.net.ConnectivityManager;
import android.net.NetworkInfo;
import androidx.annotation.NonNull;
import android.telephony.TelephonyManager;

public class NetworkUtils {
    /*
     * Keep the below constants in sync with
     * http://dxr.mozilla.org/mozilla-central/source/netwerk/base/nsINetworkLinkService.idl
     */
    public enum ConnectionSubType {
        CELL_2G("2g"),
        CELL_3G("3g"),
        CELL_4G("4g"),
        ETHERNET("ethernet"),
        WIFI("wifi"),
        WIMAX("wimax"),
        UNKNOWN("unknown");

        public final String value;
        ConnectionSubType(final String value) {
            this.value = value;
        }
    }

    /*
     * Keep the below constants in sync with
     * http://dxr.mozilla.org/mozilla-central/source/netwerk/base/nsINetworkLinkService.idl
     */
    public enum NetworkStatus {
        UP("up"),
        DOWN("down"),
        UNKNOWN("unknown");

        public final String value;

        NetworkStatus(final String value) {
            this.value = value;
        }
    }

    // Connection Type defined in Network Information API v3.
    // See Bug 1270401 - current W3C Spec (Editor's Draft) is different, it also contains wimax, mixed, unknown.
    // W3C spec: http://w3c.github.io/netinfo/#the-connectiontype-enum
    public enum ConnectionType {
        CELLULAR(0),
        BLUETOOTH(1),
        ETHERNET(2),
        WIFI(3),
        OTHER(4),
        NONE(5);

        public final int value;

        ConnectionType(final int value) {
            this.value = value;
        }
    }

    /**
     * Indicates whether network connectivity exists and it is possible to establish connections and pass data.
     */
    public static boolean isConnected(final @NonNull Context context) {
        return isConnected((ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE));
    }

    public static boolean isConnected(final ConnectivityManager connectivityManager) {
        if (connectivityManager == null) {
            return false;
        }

        final NetworkInfo networkInfo = connectivityManager.getActiveNetworkInfo();
        return networkInfo != null && networkInfo.isConnected();
    }

    /**
     * For mobile connections, maps particular connection subtype to a general 2G, 3G, 4G bucket.
     */
    public static ConnectionSubType getConnectionSubType(
            final ConnectivityManager connectivityManager) {
        if (connectivityManager == null) {
            return ConnectionSubType.UNKNOWN;
        }

        final NetworkInfo networkInfo = connectivityManager.getActiveNetworkInfo();

        if (networkInfo == null) {
            return ConnectionSubType.UNKNOWN;
        }

        switch (networkInfo.getType()) {
            case ConnectivityManager.TYPE_ETHERNET:
                return ConnectionSubType.ETHERNET;
            case ConnectivityManager.TYPE_MOBILE:
                return getGenericMobileSubtype(networkInfo.getSubtype());
            case ConnectivityManager.TYPE_WIMAX:
                return ConnectionSubType.WIMAX;
            case ConnectivityManager.TYPE_WIFI:
                return ConnectionSubType.WIFI;
            default:
                return ConnectionSubType.UNKNOWN;
        }
    }

    public static boolean isWifi(@NonNull final Context context) {
        final ConnectivityManager connectivityManager = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
        return getConnectionType(connectivityManager) == ConnectionType.WIFI;
    }

    public static ConnectionType getConnectionType(final ConnectivityManager connectivityManager) {
        if (connectivityManager == null) {
            return ConnectionType.NONE;
        }

        final NetworkInfo networkInfo = connectivityManager.getActiveNetworkInfo();
        if (networkInfo == null) {
            return ConnectionType.NONE;
        }

        switch (networkInfo.getType()) {
            case ConnectivityManager.TYPE_BLUETOOTH:
                return ConnectionType.BLUETOOTH;
            case ConnectivityManager.TYPE_ETHERNET:
                return ConnectionType.ETHERNET;
            // Fallthrough, MOBILE and WIMAX both map to CELLULAR.
            case ConnectivityManager.TYPE_MOBILE:
            case ConnectivityManager.TYPE_WIMAX:
                return ConnectionType.CELLULAR;
            case ConnectivityManager.TYPE_WIFI:
                return ConnectionType.WIFI;
            default:
                return ConnectionType.OTHER;
        }
    }

    public static NetworkStatus getNetworkStatus(final ConnectivityManager connectivityManager) {
        if (connectivityManager == null) {
            return NetworkStatus.UNKNOWN;
        }

        if (isConnected(connectivityManager)) {
            return NetworkStatus.UP;
        }
        return NetworkStatus.DOWN;
    }

    private static ConnectionSubType getGenericMobileSubtype(final int subtype) {
        switch (subtype) {
            // 2G types: fallthrough 5x
            case TelephonyManager.NETWORK_TYPE_GPRS:
            case TelephonyManager.NETWORK_TYPE_EDGE:
            case TelephonyManager.NETWORK_TYPE_CDMA:
            case TelephonyManager.NETWORK_TYPE_1xRTT:
            case TelephonyManager.NETWORK_TYPE_IDEN:
                return ConnectionSubType.CELL_2G;
            // 3G types: fallthrough 9x
            case TelephonyManager.NETWORK_TYPE_UMTS:
            case TelephonyManager.NETWORK_TYPE_EVDO_0:
            case TelephonyManager.NETWORK_TYPE_EVDO_A:
            case TelephonyManager.NETWORK_TYPE_HSDPA:
            case TelephonyManager.NETWORK_TYPE_HSUPA:
            case TelephonyManager.NETWORK_TYPE_HSPA:
            case TelephonyManager.NETWORK_TYPE_EVDO_B:
            case TelephonyManager.NETWORK_TYPE_EHRPD:
            case TelephonyManager.NETWORK_TYPE_HSPAP:
                return ConnectionSubType.CELL_3G;
            // 4G - just one type!
            case TelephonyManager.NETWORK_TYPE_LTE:
                return ConnectionSubType.CELL_4G;
            default:
                return ConnectionSubType.UNKNOWN;
        }
    }
}
