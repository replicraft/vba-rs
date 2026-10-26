' INV: vbal.6.1.2.11.1.35, func.split
' SOURCE: spec ms-vbal 6.1.2.11.1.35 Split limit (number of substrings) and Compare
' EXPECT: lim=2 [a]
' EXPECT: all=3
' EXPECT: bin=1
' EXPECT: txt=3
Public Sub Main()
    Dim a() As String
    a = Split("a,b,c", ",", 2)
    Debug.Print "lim=" & CStr(UBound(a) + 1) & " [" & a(0) & "]"
    a = Split("a,b,c", ",", -1)
    Debug.Print "all=" & CStr(UBound(a) + 1)
    a = Split("aXbxc", "x", -1, vbBinaryCompare)
    Debug.Print "bin=" & CStr(UBound(a))
    a = Split("aXbxc", "x", -1, vbTextCompare)
    Debug.Print "txt=" & CStr(UBound(a) + 1)
End Sub
