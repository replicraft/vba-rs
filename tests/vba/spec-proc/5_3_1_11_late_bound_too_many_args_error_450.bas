' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 too many positional arguments at run time raise error 450
' EXPECT: before
' ERROR: 450
'#MODULE Cls class
Public Sub M(a As Long)
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Set o = New Cls
    Debug.Print "before"
    o.M 1, 2
End Sub
