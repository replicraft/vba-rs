' INV: func.command, vbal.6.1.2.8.1.3
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/command-function
' METHOD: expected text typed by hand from the COMMAND header; Len counted by hand (/run=4, space, "a b"=5, space, -x=2 gives 13)
' COMMAND: /run "a b" -x
' EXPECT: [/run "a b" -x]
' EXPECT: [/run "a b" -x]
' EXPECT: String
' EXPECT:  8 
' EXPECT:  13 
Public Sub Main()
    Debug.Print "[" & Command & "]"
    Debug.Print "[" & Command$ & "]"
    Debug.Print TypeName(Command)
    Debug.Print VarType(Command)
    Debug.Print Len(Command)
End Sub
