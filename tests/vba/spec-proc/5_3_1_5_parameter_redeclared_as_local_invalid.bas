' INV: vbal.5.3.1.5
' SOURCE: spec ms-vbal 5.3.1.5 parameter name may not equal a local variable of the body
' ERROR: load
Sub S(x As Long)
    Dim x As Long
End Sub
Public Sub Main()
End Sub
