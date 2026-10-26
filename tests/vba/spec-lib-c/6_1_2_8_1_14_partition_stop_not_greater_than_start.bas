' INV: vbal.6.1.2.8.1.14, func.partition
' SOURCE: spec ms-vbal 6.1.2.8.1.14 Partition Stop cannot be equal to or less than Start
' EXPECT: err
Public Sub Main()
    On Error Resume Next
    Dim s As String
    s = Partition(5, 10, 10, 2)
    If Err.Number <> 0 Then Debug.Print "err" Else Debug.Print "noerr:" & s
End Sub
