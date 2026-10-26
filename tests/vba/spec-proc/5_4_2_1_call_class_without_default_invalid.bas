' INV: vbal.5.4.2.1, stmt.call
' SOURCE: spec ms-vbal 5.4.2.1 Call on a class-typed variable whose class has no default procedure is invalid
' ERROR: load
'#MODULE Plain class
Public Sub M()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim p As New Plain
    Call p
End Sub
