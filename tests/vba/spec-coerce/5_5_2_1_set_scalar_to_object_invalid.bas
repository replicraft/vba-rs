' INV: vbal.5.5.2.1, stmt.set
' SOURCE: spec ms-vbal 5.5.2.1 Set-coercion static semantics, a non-class source type is invalid for a class or Object destination
' ERROR: load
Public Sub Main()
    Dim o As Object
    Dim n As Long
    n = 5
    Set o = n
End Sub
