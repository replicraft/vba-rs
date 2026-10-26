' INV: vbal.5.5.2.2.2, stmt.set
' SOURCE: spec ms-vbal 5.5.2.2.2 Set-coercion to and from non-object types, Set of an Empty value into a class or Object is Object required
' XFAIL: wrong not implemented: error numbers of Set with a non-object Variant (13/424) differ
' ERROR: 424
Public Sub Main()
    Dim v As Variant
    Dim o As Object
    Set o = v
    Debug.Print "not reached"
End Sub
