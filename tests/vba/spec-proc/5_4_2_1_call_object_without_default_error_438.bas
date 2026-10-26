' INV: vbal.5.4.2.1, stmt.call
' SOURCE: spec ms-vbal 5.4.2.1 Call on an object with no default member raises error 438 at run time
' ERROR: 438
'#MODULE Plain class
Public Sub M()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Set o = New Plain
    Call o
End Sub
