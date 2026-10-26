' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, a cc-if-block can be nested in a cc-block; nested blocks in excluded blocks stay excluded
' EXPECT: a
' EXPECT: d
Public Sub Main()
#If True Then
  #If False Then
    Debug.Print "no1"
  #Else
    Debug.Print "a"
  #End If
#Else
  #If True Then
    Debug.Print "no2"
  #End If
#End If
#If False Then
  #If True Then
    Debug.Print "no3"
  #Else
    Debug.Print "no4"
  #End If
#Else
    Debug.Print "d"
#End If
End Sub
