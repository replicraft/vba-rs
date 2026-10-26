' INV: op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like operator
' EXPECT: False
Public Sub Main()
    Debug.Print CStr("a*b" Like "a [*]b")
End Sub
