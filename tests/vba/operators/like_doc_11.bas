' INV: op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like operator
' EXPECT: True
Public Sub Main()
    Debug.Print CStr("a [xyz" Like "a [[]*")
End Sub
