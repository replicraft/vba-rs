' INV: vbal.6.1.2.9
' SOURCE: spec ms-vbal 6.1.2.9 KeyCodeConstants numpad and function keys
' EXPECT: 96 97 98 99 100 101 102 103 104 105
' EXPECT: 106 107 108 109 110 111
' EXPECT: 112 113 114 115 116 117 118 119 120 121 122 123 124 125 126 127
Public Sub Main()
    Debug.Print vbKeyNumpad0 & " " & vbKeyNumpad1 & " " & vbKeyNumpad2 & " " & vbKeyNumpad3 & " " & vbKeyNumpad4 & " " & vbKeyNumpad5 & " " & vbKeyNumpad6 & " " & vbKeyNumpad7 & " " & vbKeyNumpad8 & " " & vbKeyNumpad9
    Debug.Print vbKeyMultiply & " " & vbKeyAdd & " " & vbKeySeparator & " " & vbKeySubtract & " " & vbKeyDecimal & " " & vbKeyDivide
    Debug.Print vbKeyF1 & " " & vbKeyF2 & " " & vbKeyF3 & " " & vbKeyF4 & " " & vbKeyF5 & " " & vbKeyF6 & " " & vbKeyF7 & " " & vbKeyF8 & " " & vbKeyF9 & " " & vbKeyF10 & " " & vbKeyF11 & " " & vbKeyF12 & " " & vbKeyF13 & " " & vbKeyF14 & " " & vbKeyF15 & " " & vbKeyF16
End Sub
