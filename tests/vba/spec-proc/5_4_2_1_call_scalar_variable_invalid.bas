' INV: vbal.5.4.2.1, stmt.call
' SOURCE: spec ms-vbal 5.4.2.1 Call on a variable whose declared type is not a class or Object is invalid
' ERROR: load
Public Sub Main()
    Dim n As Long
    Call n
End Sub
