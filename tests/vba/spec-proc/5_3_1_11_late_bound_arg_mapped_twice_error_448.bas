' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 two arguments mapped to one parameter raise error 448 at run time
' XFAIL: wrong not implemented: error numbers of late-bound named/omitted arguments
' ERROR: 448
'#MODULE Cls class
Public Sub M(a As Long)
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Set o = New Cls
    o.M 1, a:=2
End Sub
