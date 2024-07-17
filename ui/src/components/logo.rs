use dioxus::prelude::*;

pub fn Logo() -> Element {
    rsx! {
        svg {
            width: "859",
            height: "168",
            view_box: "0 0 859 168",
            fill: "none",
            style: "height: 24px; width: 120px",
            xmlns: "http://www.w3.org/2000/svg",
            g {
                clip_path: "url(#clip0_26_10877)",
                path {
                    d: "M77.7253 113.135L55.6353 69.7051L77.7253 113.135Z",
                    fill: "#97B398"
                },
                path {
                     d:" M77.725 113.135L55.635 69.7051H31.415L0.665039 130.135H22.205L39.515 96.135C39.8936 95.4005 40.4671 94.7844 41.1727 94.3542C41.8783 93.9241 42.6887 93.6967 43.515 93.6967C44.3414 93.6967 45.1518 93.9241 45.8574 94.3542C46.563 94.7844 47.1365 95.4005 47.515 96.135L64.815 130.135H86.375L77.725 113.135Z",
                     fill: "#97B398"
                },
                path {
                    d: "M113.015 43.6751L90.9248 0.245117L113.015 43.6751Z",
                    fill: "#97B398"
                },
                path {
                    d: "M113.025 43.6752L90.9248 0.245117H66.7048L35.9648 60.6752H57.5048L74.8048 26.6752C75.1834 25.9406 75.7569 25.3245 76.4625 24.8944C77.1681 24.4643 77.9785 24.2367 78.8048 24.2367C79.6312 24.2367 80.4416 24.4643 81.1472 24.8944C81.8528 25.3245 82.4263 25.9406 82.8048 26.6752L100.115 60.6752H121.675L113.025 43.6752Z",
                    fill: "#97B398"
                },
                path {
                    d: "M148.355 113.135L126.255 69.7051L148.355 113.135Z",
                    fill: "#97B398"
                },
                path {
                    d: "M148.355 113.135L126.265 69.7051H102.035L71.2949 130.135H92.8349L110.135 96.135C110.516 95.4034 111.09 94.7903 111.796 94.3624C112.501 93.9346 113.31 93.7084 114.135 93.7084C114.96 93.7084 115.769 93.9346 116.474 94.3624C117.179 94.7903 117.754 95.4034 118.135 96.135L135.435 130.135H157.005L148.355 113.135Z",
                    fill: "#97B398"
                },
                path {
                    d: "M621.685 36.0552C594.785 36.0552 576.035 54.7951 576.035 83.3751V85.6052C576.035 113.995 594.595 132.735 622.435 132.735C651.435 132.735 665.225 113.405 665.225 100.075H647.485C643.585 110.305 637.275 116.075 622.805 116.075C607.395 116.075 596.085 105.865 595.525 89.7252H665.855V82.4851C665.855 54.6151 648.035 36.0552 621.685 36.0552ZM595.685 75.0552C597.545 61.3252 607.005 52.9752 621.485 52.9752C635.775 52.9752 645.415 61.3252 646.535 75.0552H595.685Z",
                    fill: "#97B398"
                },
                path {
                    d: "M693.365 0.245117H674.245V130.145H693.365V0.245117Z",
                    fill: "#97B398"
                },
                path {
                    d: "M858.335 0.245117H839.215V130.145H858.335V0.245117Z",
                    fill: "#97B398"
                },
                path {
                    d: "M716.405 0.245117C708.795 0.245117 703.235 5.43512 703.235 13.2451C703.235 21.0551 708.795 26.2451 716.405 26.2451C723.825 26.2451 729.585 21.0451 729.585 13.2451C729.585 5.44512 723.825 0.245117 716.405 0.245117Z",
                    fill: "#97B398"
                },
                path {
                    d: "M725.865 38.6553H706.755V130.135H725.865V38.6553Z",
                    fill: "#97B398"
                },
                path {
                    d: "M449.575 85.0452C449.575 104.715 439.375 115.475 423.575 115.475C409.475 115.475 401.125 108.245 401.125 92.2852V38.6552H381.995V93.7651C381.995 117.335 396.995 131.765 417.995 131.765C429.675 131.765 437.095 128.275 441.735 123.765C442.613 122.966 443.758 122.524 444.945 122.525C446.213 122.525 447.429 123.029 448.325 123.925C449.221 124.822 449.725 126.037 449.725 127.305V144.905C449.725 148.615 448.055 150.475 444.525 150.475H393.525V167.175H450.305C461.625 167.175 468.675 160.175 468.675 148.805V38.5752H449.575V85.0452Z",
                    fill: "#97B398"
                },
                path {
                    d: "M307.405 103.625C307.182 104.704 306.593 105.673 305.739 106.369C304.885 107.065 303.817 107.445 302.715 107.445C301.643 107.445 300.602 107.084 299.76 106.42C298.918 105.756 298.324 104.828 298.075 103.785L272.845 0.245117H235.175V130.135H254.655V42.2052C254.655 40.9295 255.162 39.706 256.064 38.804C256.966 37.9019 258.189 37.3951 259.465 37.3951C260.528 37.3957 261.56 37.7483 262.402 38.3978C263.243 39.0473 263.845 39.9571 264.115 40.9851L285.835 130.135H319.575L341.295 40.9752C341.566 39.9507 342.168 39.0442 343.007 38.3969C343.846 37.7495 344.875 37.3974 345.935 37.3951C347.21 37.3951 348.434 37.9019 349.336 38.804C350.238 39.706 350.745 40.9295 350.745 42.2052V130.135H370.235V0.245117H332.575L307.405 103.625Z",
                    fill: "#97B398"
                },
                path {
                    d: "M524.745 53.1351C540.885 53.1351 548.865 62.4052 551.095 75.2152H569.655C569.655 57.8952 555.155 36.0552 524.375 36.0552C497.835 36.0552 477.425 53.5052 477.425 83.1952V85.6052C477.425 115.295 497.835 132.735 524.375 132.735C555.155 132.735 569.655 110.905 569.655 93.5852H551.095C548.865 106.385 540.885 115.665 524.745 115.665C508.745 115.665 496.535 104.535 496.535 85.2351V83.5452C496.575 64.2652 508.785 53.1351 524.745 53.1351Z",
                    fill: "#97B398"
                },
                path {
                    d: "M823.735 114.175C820.395 114.175 818.535 112.325 818.535 108.615V70.9452C818.535 48.6752 804.065 36.0552 780.315 36.0552C761.075 36.0552 749.515 44.4751 743.535 55.5151C741.802 58.8145 740.46 62.3047 739.535 65.9152H759.045C761.835 57.6052 768.265 52.0151 780.145 52.0151C793.505 52.0151 799.815 59.0151 799.815 69.6451V75.5852H771.965C751.965 75.5852 737.075 85.2352 737.075 103.785C737.075 122.335 751.925 132.785 771.405 132.785C784.975 132.785 792.235 127.785 796.285 122.785C796.734 122.241 797.297 121.803 797.935 121.502C798.573 121.201 799.27 121.045 799.975 121.045C800.847 121.046 801.701 121.285 802.447 121.736C803.192 122.187 803.8 122.833 804.205 123.605V123.475C806.435 127.645 811.205 130.125 817.205 130.125H831.575V114.125L823.735 114.175ZM799.795 92.2852C799.795 107.685 789.405 116.965 774.195 116.965C762.875 116.965 756.195 111.585 756.195 103.235C756.195 94.8851 762.875 90.4252 773.075 90.4252H799.795V92.2852Z",
                    fill: "#97B398"
                },
            }
            defs {
                clipPath{
                     id: "clip0_26_10877",
                     rect {
                          width: "857.67",
                          height: "167.01",
                          fill: "white",
                          transform: "translate(0.665039 0.245117)"
                     },
                }
            }
        }
    }
}

pub fn LogoDark() -> Element {
    rsx! {
        svg {
            width: "135",
            height: "27",
            view_box: "0 0 135 27",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            path {
                 fill_rule: "evenodd",
                 clip_rule: "evenodd",
                 d: "M17.6857 7.02457L14.2071 0H10.3948L5.55619 9.77422H8.94666L11.6697 4.27492C11.7293 4.1561 11.8196 4.05645 11.9307 3.98688C12.0417 3.91732 12.1693 3.8805 12.2993 3.8805C12.4294 3.8805 12.557 3.91732 12.668 3.98688C12.7791 4.05645 12.8694 4.1561 12.929 4.27492L15.6536 9.77422H19.0472L17.6857 7.02457ZM12.1295 18.2593L8.65246 11.2348H4.84015L0 21.0089H3.39046L6.11512 15.5096C6.17471 15.3908 6.26498 15.2912 6.37604 15.2216C6.48711 15.152 6.61467 15.1153 6.74473 15.1153C6.87481 15.1153 7.00237 15.152 7.11343 15.2216C7.22449 15.2912 7.31476 15.3908 7.37434 15.5096L10.0974 21.0089H11.1174H13.491H14.5079L17.231 15.5096C17.2909 15.3913 17.3813 15.2921 17.4924 15.2229C17.6034 15.1537 17.7307 15.1172 17.8606 15.1172C17.9904 15.1172 18.1178 15.1537 18.2288 15.2229C18.3399 15.2921 18.4302 15.3913 18.4902 15.5096L21.2133 21.0089H24.6085L23.2469 18.2593L19.7699 11.2348H15.956L12.3041 18.6119L12.1295 18.2593ZM90.5651 13.4458C90.5651 8.82318 93.5164 5.79211 97.7505 5.79211C101.898 5.79211 104.703 8.79406 104.703 13.3019V14.4729H93.6329C93.721 17.0834 95.5012 18.7349 97.9268 18.7349C100.204 18.7349 101.198 17.8016 101.812 16.1469H104.604C104.604 18.303 102.433 21.4295 97.8686 21.4295C93.4865 21.4295 90.5651 18.3984 90.5651 13.8065V13.4458ZM97.719 8.52882C95.4398 8.52882 93.9508 9.87938 93.658 12.1001H101.662C101.486 9.87938 99.9683 8.52882 97.719 8.52882ZM109.033 0H106.024V21.0106H109.033V0ZM131.99 0H135V21.0106H131.99V0ZM112.66 0C111.462 0 110.587 0.839453 110.587 2.10267C110.587 3.3659 111.462 4.20535 112.66 4.20535C113.828 4.20535 114.735 3.36428 114.735 2.10267C114.735 0.841071 113.828 0 112.66 0ZM111.141 6.21259H114.149V21.0089H111.141V6.21259ZM70.6599 13.716C70.6599 16.8974 69.0543 18.6378 66.5674 18.6378C64.348 18.6378 63.0337 17.4684 63.0337 14.887V6.21265H60.0225V15.1264C60.0225 18.9387 62.3836 21.2726 65.689 21.2726C67.5275 21.2726 68.6954 20.7081 69.4258 19.9787C69.564 19.8494 69.7442 19.778 69.9311 19.7781C70.1307 19.7781 70.3221 19.8596 70.4631 20.0046C70.6041 20.1496 70.6835 20.3462 70.6835 20.5513V23.398C70.6835 23.998 70.4206 24.2989 69.865 24.2989H61.8374V27H70.7747C72.5566 27 73.6663 25.8678 73.6663 24.0288V6.19971H70.6599V13.716ZM48.0197 17.1649C48.1543 17.0524 48.2469 16.8956 48.282 16.7211L52.2438 0H58.1716V21.009H55.1038V6.78681C55.1038 6.58047 55.024 6.38257 54.882 6.23668C54.7401 6.09077 54.5476 6.0088 54.3467 6.0088C54.1799 6.00917 54.0179 6.06612 53.8858 6.17083C53.7538 6.27553 53.659 6.42215 53.6164 6.58786L50.1976 21.009H44.8868L41.468 6.58946C41.4255 6.42319 41.3307 6.27603 41.1983 6.17098C41.066 6.06593 40.9034 6.0089 40.736 6.0088C40.5352 6.0088 40.3427 6.09077 40.2007 6.23668C40.0587 6.38257 39.9789 6.58047 39.9789 6.78681V21.009H36.9127V0H42.8421L46.8134 16.747C46.8527 16.9157 46.9461 17.0658 47.0786 17.1732C47.2111 17.2806 47.375 17.339 47.5437 17.339C47.7172 17.339 47.8853 17.2775 48.0197 17.1649ZM82.492 8.55468C85.0325 8.55468 86.2886 10.0541 86.6396 12.126H89.561C89.561 9.3246 87.2787 5.79211 82.4338 5.79211C78.2563 5.79211 75.0437 8.61454 75.0437 13.4167V13.8065C75.0437 18.6087 78.2563 21.4295 82.4338 21.4295C87.2787 21.4295 89.561 17.8986 89.561 15.0973H86.6396C86.2886 17.1676 85.0325 18.6685 82.492 18.6685C79.9736 18.6685 78.0517 16.8683 78.0517 13.7467V13.4733C78.058 10.3549 79.9799 8.55468 82.492 8.55468ZM128.735 17.5282C128.735 18.1283 129.028 18.4275 129.554 18.4275L130.788 18.4195V21.0074H128.526C127.582 21.0074 126.831 20.6062 126.48 19.9318V19.9528C126.416 19.8279 126.32 19.7234 126.203 19.6505C126.086 19.5775 125.951 19.5389 125.814 19.5387C125.703 19.5387 125.593 19.564 125.493 19.6126C125.392 19.6613 125.304 19.7322 125.233 19.8202C124.596 20.6289 123.453 21.4376 121.317 21.4376C118.251 21.4376 115.913 19.7474 115.913 16.747C115.913 13.7467 118.257 12.1859 121.405 12.1859H125.789V11.2251C125.789 9.50574 124.796 8.37353 122.693 8.37353C120.823 8.37353 119.811 9.2777 119.371 10.6218H116.301C116.446 10.0378 116.657 9.47329 116.93 8.93963C117.871 7.15398 119.691 5.79211 122.719 5.79211C126.458 5.79211 128.735 7.83332 128.735 11.4354V17.5282ZM121.756 18.8788C124.15 18.8788 125.786 17.3778 125.786 14.887V14.5861H121.58C119.974 14.5861 118.923 15.3075 118.923 16.6581C118.923 18.0086 119.974 18.8788 121.756 18.8788Z",
                 fill: "#3A554C"
            }
        }
    }
}
