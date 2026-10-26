' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 at least one enum-element must be an enum-member
' ERROR: load
' STRICT_SPEC: true
Private Enum Bad
    Rem nothing here
End Enum
Public Sub Main()
End Sub
