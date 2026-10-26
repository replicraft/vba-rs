' INV: vbal.5.4.2.5, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.5 Exit For inside only a Do loop is not inside a For
' ERROR: load
Public Sub Main()
    Do
        Exit For
    Loop
End Sub
