' INV: obj.err
' SOURCE: spec ms-vbal 6.1.3.2.1.1 statements that clear the Err object, 5.4.4 Resume Next policy
' EXPECT:  5 
Sub Inner()
    On Error Resume Next
    Error 5
End Sub
Public Sub Main()
    Inner
    Debug.Print Err.Number
End Sub
