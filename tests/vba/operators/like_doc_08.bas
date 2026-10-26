' INV: op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like operator
' EXPECT: True
Public Sub Main()
    Debug.Print CStr("ab" Like "a*b")
End Sub
