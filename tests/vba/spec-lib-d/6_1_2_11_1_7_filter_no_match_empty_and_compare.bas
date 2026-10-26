' INV: vbal.6.1.2.11.1.7, func.filter
' SOURCE: spec ms-vbal 6.1.2.11.1.7 Filter no match gives empty array; Compare text vs binary
' EXPECT: none ub=-1
' EXPECT: bin=1
' EXPECT: txt=2
Public Sub Main()
    Dim src As Variant, r As Variant
    src = Array("ABC", "abc", "xyz")
    r = Filter(src, "qq")
    Debug.Print "none ub=" & CStr(UBound(r))
    r = Filter(src, "abc", True, vbBinaryCompare)
    Debug.Print "bin=" & CStr(UBound(r) - LBound(r) + 1)
    r = Filter(src, "abc", True, vbTextCompare)
    Debug.Print "txt=" & CStr(UBound(r) - LBound(r) + 1)
End Sub
