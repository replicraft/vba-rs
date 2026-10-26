' INV: vbal.5.4.2.10, stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 with no match and no Case Else nothing executes
' EXPECT: after
Public Sub Main()
    Select Case 3
        Case 1
            Debug.Print "one"
        Case 2
            Debug.Print "two"
    End Select
    Debug.Print "after"
End Sub
