' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 ByVal parameter requires a valid Let-coercion from the argument type
' ERROR: load
Type Pt
    X As Long
End Type
Sub S(ByVal n As Long)
End Sub
Public Sub Main()
    Dim p As Pt
    S p
End Sub
