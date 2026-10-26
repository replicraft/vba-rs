' INV: vbal.6.1.2.9
' SOURCE: spec ms-vbal 6.1.2.9 KeyCodeConstants letter and digit keys
' EXPECT: 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80 81 82 83 84 85 86 87 88 89 90
' EXPECT: 48 49 50 51 52 53 54 55 56 57
Public Sub Main()
    Debug.Print vbKeyA & " " & vbKeyB & " " & vbKeyC & " " & vbKeyD & " " & vbKeyE & " " & vbKeyF & " " & vbKeyG & " " & vbKeyH & " " & vbKeyI & " " & vbKeyJ & " " & vbKeyK & " " & vbKeyL & " " & vbKeyM & " " & vbKeyN & " " & vbKeyO & " " & vbKeyP & " " & vbKeyQ & " " & vbKeyR & " " & vbKeyS & " " & vbKeyT & " " & vbKeyU & " " & vbKeyV & " " & vbKeyW & " " & vbKeyX & " " & vbKeyY & " " & vbKeyZ
    Debug.Print vbKey0 & " " & vbKey1 & " " & vbKey2 & " " & vbKey3 & " " & vbKey4 & " " & vbKey5 & " " & vbKey6 & " " & vbKey7 & " " & vbKey8 & " " & vbKey9
End Sub
