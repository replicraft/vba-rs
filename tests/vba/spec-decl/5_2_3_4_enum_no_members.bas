' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 at least one enum-element must be an enum-member
' XFAIL: wrong pending
' ERROR: load
Private Enum Bad
    Rem nothing here
End Enum
Public Sub Main()
End Sub
