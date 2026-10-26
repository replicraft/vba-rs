' INV: stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 Select Case statement expression range-clause equality
' EXPECT: lt10
Public Sub Main()
    Dim x As Long
    x = 7
    Select Case True
        Case x < 5
            Debug.Print "lt5"
        Case x < 10
            Debug.Print "lt10"
    End Select
End Sub
