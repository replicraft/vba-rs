' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 missing non-optional argument raises error 449 at run time
' ERROR: 449
'#MODULE Cls class
Public Sub M(a As Long, b As Long)
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Set o = New Cls
    o.M 1
End Sub
