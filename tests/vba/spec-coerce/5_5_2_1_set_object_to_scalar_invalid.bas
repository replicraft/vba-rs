' INV: vbal.5.5.2.1, stmt.set
' SOURCE: spec ms-vbal 5.5.2.1 Set-coercion static semantics, a destination that is not a class, Object or Variant is invalid
' ERROR: load
Public Sub Main()
    Dim o As Object
    Dim n As Long
    Set o = New Collection
    Set n = o
End Sub
