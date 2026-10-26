' INV: stmt.select-case
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
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
