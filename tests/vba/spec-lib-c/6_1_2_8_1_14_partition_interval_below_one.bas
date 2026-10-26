' INV: vbal.6.1.2.8.1.14, func.partition
' SOURCE: spec ms-vbal 6.1.2.8.1.14 Partition Interval cannot be less than 1
' EXPECT: err
Public Sub Main()
    On Error Resume Next
    Dim s As String
    s = Partition(5, 0, 10, 0)
    If Err.Number <> 0 Then Debug.Print "err" Else Debug.Print "noerr:" & s
End Sub
