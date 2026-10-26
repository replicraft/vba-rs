' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 object-typed ByVal parameter rejects a scalar-typed argument
' XFAIL: wrong not implemented: static rejection of a scalar passed to a ByVal Object parameter
' ERROR: load
Sub S(ByVal o As Object)
End Sub
Public Sub Main()
    Dim n As Long
    S n
End Sub
