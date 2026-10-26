' INV: vbal.5.6.3
' SOURCE: spec ms-vbal 5.6.3 Member resolution, a class target without the named member raises error 438
' ERROR: 438
'#MODULE Plain class
Public Sub Hello()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Dim x As Variant
    Set o = New Plain
    x = o.Missing
    Debug.Print "not reached"
End Sub
