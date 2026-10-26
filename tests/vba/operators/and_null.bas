' INV: op.and, lit.null
' SOURCE: spec ms-vbal 5.6.9.8.2 And operator (Null)
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(Null And False)
    Debug.Print IsNull(Null And True)
End Sub
