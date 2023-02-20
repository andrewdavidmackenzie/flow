#compile sample
flowc -C flowr/src/cli -c -d flowsamples/sequence-of-sequences
while true;
do
  # run the sample, sending number sequence from stdout to result.txt and logs to log.txt
  flowr -v debug -t 4 -n flowsamples/sequence-of-sequences > result.txt 2> log.txt
  diff result.txt success.txt
  if [ $? -eq 1 ]
  then
    echo "result.txt does not match success.txt, see log.txt for logs"
    exit 1
  else
    echo "passed"
  fi
done
