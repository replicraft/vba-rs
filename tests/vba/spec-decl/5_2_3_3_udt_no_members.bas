' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 at least one udt-element must be a udt-member
' XFAIL: wrong pending
' ERROR: load
Private Type Empty1
    Rem only a comment
End Type
Public Sub Main()
End Sub
