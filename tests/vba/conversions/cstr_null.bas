' INV: func.cstr, lit.null
' SOURCE: spec ms-vbal 5.5.1.2.10 Let-coercion from Null
' ERROR: 94
Public Sub Main()
    Debug.Print CStr(Null)
End Sub
