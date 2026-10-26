' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like uses module Option Compare (binary: case sensitive, ranges by code)
' EXPECT: False
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr("ABC" Like "abc")
    Debug.Print CStr("B" Like "[a-c]")
    Debug.Print CStr("b" Like "[a-c]")
End Sub
