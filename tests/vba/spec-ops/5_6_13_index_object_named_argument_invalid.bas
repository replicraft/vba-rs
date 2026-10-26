' INV: vbal.5.6.13, expr.index
' SOURCE: spec ms-vbal 5.6.13 index on Object or Variant allows no named arguments
' XFAIL: wrong not implemented: static rejection of named arguments in an index expression on an Object
' ERROR: load
Public Sub Main()
    Dim o As Object
    Debug.Print CStr(o(k:=1))
End Sub
