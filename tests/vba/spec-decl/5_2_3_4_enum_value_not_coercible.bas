' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum member value MUST be coercible to Long
' ERROR: load
Private Enum Bad
    P = "abc"
End Enum
Public Sub Main()
End Sub
