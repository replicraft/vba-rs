' INV: stmt.goto, lex.label
' SOURCE: spec ms-vbal 5.4.2.12 GoTo statement, 5.4.1.1 line-number labels
' EXPECT: twenty
Public Sub Main()
    GoTo 20
    10 Debug.Print "ten"
    20 Debug.Print "twenty"
End Sub
