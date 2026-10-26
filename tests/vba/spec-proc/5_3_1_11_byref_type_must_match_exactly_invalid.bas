' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 ByRef parameter requires exact declared-type match of the argument
' ERROR: load
Sub S(ByRef x As Long)
End Sub
Public Sub Main()
    Dim i As Integer
    S i
End Sub
