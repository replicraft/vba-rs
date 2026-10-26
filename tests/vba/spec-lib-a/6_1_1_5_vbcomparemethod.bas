' INV: vbal.6.1.1.5
' SOURCE: spec ms-vbal 6.1.1.5 VbCompareMethod values
' EXPECT: vbBinaryCompare=0
' EXPECT: vbTextCompare=1
Public Sub Main()
    Debug.Print "vbBinaryCompare=" & CStr(vbBinaryCompare)
    Debug.Print "vbTextCompare=" & CStr(vbTextCompare)
End Sub
