' INV: vbal.5.5.2.2.2, stmt.set
' SOURCE: spec ms-vbal 5.5.2.2.2 Set-coercion to and from non-object types, Set of a non-object value into a Variant is Type mismatch
' XFAIL: wrong not implemented: error numbers of Set with a non-object Variant (13/424) differ
' ERROR: 13
Public Sub Main()
    Dim v As Variant
    Dim w As Variant
    v = 5
    Set w = v
    Debug.Print "not reached"
End Sub
