' INV: func.replace
' SOURCE: spec ms-vbal 6.1.2.11.1.29 Replace Compare argument and 5.6.9.5 string comparison modes
' EXPECT: xxx
' EXPECT: xAx
Public Sub Main()
    Debug.Print CStr(Replace("aAa", "a", "x", 1, -1, vbTextCompare))
    Debug.Print CStr(Replace("aAa", "a", "x", 1, -1, vbBinaryCompare))
End Sub
