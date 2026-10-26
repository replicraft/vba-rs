' INV: vbal.5.6.9.5, op.eq, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational Integer effective type: Integer vs String/Empty, Empty vs Empty
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim a As Variant, b As Variant
    Debug.Print CStr(CInt(7) = "7")
    Debug.Print CStr(CInt(7) < "10")
    Debug.Print CStr(a = b)
    Debug.Print CStr(a < b)
    Debug.Print CStr(a <= b)
    Debug.Print CStr(CInt(300) > CByte(200))
End Sub
