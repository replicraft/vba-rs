' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 omitted positional value for non-optional parameter raises error 448 at run time
' XFAIL: wrong pending
' ERROR: 448
'#MODULE Cls class
Public Sub M(a As Long, b As Long)
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Set o = New Cls
    o.M , 2
End Sub
