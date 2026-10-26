' INV: vbal.5.3.1.6
' SOURCE: spec ms-vbal 5.3.1.6 procedure name may not equal an enum member name
' ERROR: load
Enum Kind
    Alpha = 1
End Enum
Sub Alpha()
End Sub
Public Sub Main()
End Sub
