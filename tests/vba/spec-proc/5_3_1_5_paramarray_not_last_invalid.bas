' INV: vbal.5.3.1.5, decl.paramarray
' SOURCE: spec ms-vbal 5.3.1.5 parameter-list grammar: ParamArray comes last
' ERROR: load
Sub S(ParamArray p() As Variant, b As Long)
End Sub
Public Sub Main()
End Sub
