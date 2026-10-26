' INV: vbal.5.6.9.5, op.lt, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational String effective type, binary compare mode and Empty
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: True
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Dim e As Variant
    Debug.Print CStr("B" < "a")
    Debug.Print CStr("a" < "b")
    Debug.Print CStr("ab" > "a")
    Debug.Print CStr("a" = "A")
    Debug.Print CStr("" < "a")
    Debug.Print CStr(e = "")
    Debug.Print CStr(e = "0")
    Debug.Print CStr("0" > e)
End Sub
