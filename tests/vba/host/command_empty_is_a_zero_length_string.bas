' INV: func.command, vbal.6.1.2.8.1.3
' SOURCE: derived Command returns the argument portion of the command line; with none it is a zero-length String, not Empty or Null
' METHOD: by hand: VarType 8 is vbString, Len of a zero-length string is 0
' EXPECT:  8 
' EXPECT:  0 
' EXPECT: True
Public Sub Main()
    Debug.Print VarType(Command)
    Debug.Print Len(Command)
    Debug.Print Command = ""
End Sub
