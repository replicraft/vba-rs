' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 named argument without a matching parameter raises error 448 at run time
' ERROR: 448
'#MODULE Cls class
Public Sub M(a As Long)
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Set o = New Cls
    o.M zzz:=1
End Sub
