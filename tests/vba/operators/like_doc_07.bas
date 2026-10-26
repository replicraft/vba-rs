' INV: op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like operator
' EXPECT: False
Public Sub Main()
    Debug.Print CStr("CAT123khg" Like "B?T*")
End Sub
